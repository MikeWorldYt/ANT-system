import { CommonModule } from '@angular/common';
import { Component, HostListener, AfterViewInit, OnInit } from '@angular/core';
import { NavigationEnd, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { ActivatedRoute, Router } from '@angular/router';
import { IntersectionService } from '../../services/IntersectionObserver.service';
import { TitleStateService } from '../../services/title.service';

@Component({
  selector: 'app-nav',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    RouterLink,
    RouterLinkActive
  ],
  templateUrl: './nav.component.html',
  styleUrl: './nav.component.css'
})
export class NavComponent implements OnInit, AfterViewInit{
  // ▲ SERVICES ▲
  constructor(
    private route: ActivatedRoute,
    private router: Router,
    private intersectionService: IntersectionService,
    private titleStateService: TitleStateService
  ) { }

  // Toogle
  currTitle: string = ''; // default property
  showMe: boolean = true;
  visible: boolean = false;

  // ████ Toggle Title ████ ƒ
  toggleTitle(section: string) {
    if (this.currTitle === section) {
      this.currTitle = ''; // Si se hace clic en la misma sección, la ocultamos
      console.log(`s ${this.currTitle} se ha cerrado`);
    } else {
      this.currTitle = section; // Si se hace clic en una nueva sección, la mostramos
      console.log(`s ${this.currTitle} se ha abierto`);
    }
    if (this.currTitle === 'concepts') {
      this.moduleSection = 'levels-and-structure';
    }
  }

  // Module <section> control
  moduleSection: string = '';
  hovered: boolean = false;

  //  ▬▬▬ Intersection Section <section>
  idActive: string = '';

  ngOnInit() {
    this.router.events.subscribe(event => {
      if (event instanceof NavigationEnd) {
        const urlTree = this.router.parseUrl(this.router.url);
        const fragment = urlTree.fragment;
        if (fragment) {
          const element = document.getElementById(fragment);
          if (element) {
            element.scrollIntoView({ behavior: 'smooth' });
          }
        }
      }
    });
    // Titles Toggle
    this.titleStateService.currTitle$.subscribe(section => {
      this.currTitle = section;
    })
  }

  ngAfterViewInit() {
    this.route.fragment.subscribe(fragment => {
      if (fragment) {
        this.moduleSection = fragment;
        // this.scrollToSection(fragment);
      }
    });

    // Intersection Observer

    this.intersectionService.getCurrentId().subscribe(id => {
      this.idActive = id;
    })
  }

  @HostListener('document:scroll', [])
onWindowScroll() {
  console.log('Scrolling...');
  const sections = document.querySelectorAll('h2');
  sections.forEach(section => {
    const rect = section.getBoundingClientRect();
    if (rect.top >= 0 && rect.bottom <= (window.innerHeight || document.documentElement.clientHeight)) {
      this.moduleSection = section.id;
    }
  }); 
}

  scrollToSection(sectionId: string) {
    document.getElementById(sectionId)?.scrollIntoView({ behavior: 'smooth' });
    this.moduleSection = sectionId;
    console.log(`SectionId: ${sectionId}`);

  }

  moduletoggle(section: string) {
    this.moduleSection = section;
  }

  onAnchorClick(event: Event, sectionId: string) {
    event.preventDefault();
    this.scrollToSection(sectionId);
    this.router.navigate([], { fragment: sectionId });
  }
}

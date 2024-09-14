import { CommonModule } from '@angular/common';
import { Component, HostListener, AfterViewInit, OnInit, ChangeDetectorRef } from '@angular/core';
import { NavigationEnd, RouterLink, RouterLinkActive, RouterOutlet } from '@angular/router';
import { ActivatedRoute, Router } from '@angular/router';

// ▲ SERVICES ▲
import { ArticleService } from '../../services/navArticleObserver.service';
import { TitleService } from '../../services/navTitle.service';
import { LanguageService } from '../../services/navLanguage.service';
import { PageService } from '../../services/navPage.service';

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
    private languageService: LanguageService,
    private TitleService: TitleService,
    private pageService: PageService,
    private intersectionService: ArticleService,
    private cdr: ChangeDetectorRef
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: string = '';
  currentTitle: string = '';
  currentPage: string = '';
  currentArticle: string = '';

  // ████ OnInit
  ngOnInit(): void {
    // Subscribe to Language Service
    this.languageService.currentLanguage$.subscribe(language => {
      this.currentLanguage = language;
      this.cdr.detectChanges();
    });
    // TO DO: REVIEW THIS
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
    // Subsribe to Title Service
    this.TitleService.currentTitle$.subscribe(title => {
      this.currentTitle = title;
      this.cdr.detectChanges();
    })
  }

  // ████ AfterViewInit
  ngAfterViewInit() {
    this.route.fragment.subscribe(page => {
      if (page) {
        this.currentPage = page;
      }
    });
  // Intersection Observer function
    this.intersectionService.getcurrentArticle().subscribe(id => {
      this.currentArticle = id;
    })
  }

  // Toogle
  showMe: boolean = true;
  visible: boolean = false;

  // ████ Toggle Title ████ ƒ
  toggleTitle(section: string) {
    if (this.currentTitle === section) {
      this.currentTitle = ''; // Si se hace clic en la misma sección, la ocultamos
    } else {
      this.currentTitle = section; // Si se hace clic en una nueva sección, la mostramos
    }
    if (this.currentTitle === 'concepts') {
      this.currentPage = 'levels-and-structure';
    }
  }

  // Module <section> control
  hovered: boolean = false;

  @HostListener('document:scroll', [])
onWindowScroll() {
  console.log('Scrolling...');
  const sections = document.querySelectorAll('h2');
  sections.forEach(section => {
    const rect = section.getBoundingClientRect();
    if (rect.top >= 0 && rect.bottom <= (window.innerHeight || document.documentElement.clientHeight)) {
      this.currentPage = section.id;
    }
  }); 
}

  navPageToggle(page: string) {
    this.pageService.setCurrentPage(page);
  }

  onAnchorClick(event: Event, page: string) {
    event.preventDefault();
    this.router.navigate([], { fragment: page });
  }

}
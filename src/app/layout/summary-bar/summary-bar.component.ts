import { AfterViewInit, Component, OnInit } from '@angular/core';
import { NgClass } from '@angular/common';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/lenguaje.service';
import { IntersectionService } from '../../services/IntersectionObserver.service';
import { Language } from '../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../docs/content/content';

@Component({
  selector: 'app-summary-bar',
  standalone: true,
  imports: [
    NgClass
  ],
  templateUrl: './summary-bar.component.html',
  styleUrl: './summary-bar.component.css'
})
export class SummaryBarComponent implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService
  ) { }

  //  ▬▬▬ Intersection Section <section>
  idActive: string = '';

  // ████ AfterViewInit

  ngOnInit(): void {

  }

  ngAfterViewInit() {
    // Intersection Observer function
    this.intersectionService.getCurrentId().subscribe(id => {
      this.idActive = id;
    })
  }

  // ▬▬▬ Language
  currentLanguage: Language = 'ES';

  // ███ Language Controller ███

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }


}
